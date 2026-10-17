//! Wavefront OBJ geometry: `v`, `vt` and `f` statements.
//!
//! Every `v` line is a soup vertex, in file order, used or not. A corner's provenance
//! is its primitive and its position index. A primitive is a maximal run of `f` lines
//! with no `o`, `g` or `usemtl` line between them, numbered from 0 in file order, so
//! every primitive has one material: the name on the last `usemtl` line, kept across
//! `o` and `g`. No MTL file is read. Normals, lines and points are not read;
//! free-form geometry is refused.

mod parse;

use openplush_geom::DVec2;
use openplush_mesh::{MaterialIndex, SourceCorner, SourceMaterials, TriangleSoup};

use crate::triangulate::triangulate;
use crate::{Format, IoError};
use parse::{Face, LineError, Statements};

/// Reads OBJ bytes into a soup, triangulating polygons.
///
/// Errors: [`IoError::Parse`] for text that is not UTF-8, more vertices than a `u32`
/// can index, or a statement it cannot read (its detail then starts `line N:`),
/// including a polygon that crosses itself; [`IoError::NonFiniteInput`] naming every
/// vertex with a NaN or infinite coordinate.
pub fn read(bytes: &[u8]) -> Result<TriangleSoup, IoError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|e| parse_error(format!("byte {}: not UTF-8 text", e.valid_up_to())))?;
    // A byte-order mark is not whitespace; left in, it would hide the first statement.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let statements = parse::statements(text).map_err(located)?;
    if u32::try_from(statements.positions.len()).is_err() {
        let count = statements.positions.len();
        return Err(parse_error(format!(
            "{count} vertices exceed the u32 index space"
        )));
    }
    check_ranges(&statements).map_err(located)?;
    refuse_non_finite(&statements)?;
    build(statements)
}

fn parse_error(detail: String) -> IoError {
    IoError::Parse {
        format: Format::Obj,
        detail,
    }
}

fn located(e: LineError) -> IoError {
    parse_error(format!("line {}: {}", e.line, e.reason))
}

/// Positive indices may point forward, so ranges are checked once every line is read.
fn check_ranges(s: &Statements) -> Result<(), LineError> {
    for face in &s.faces {
        for c in &face.corners {
            if c.vertex >= s.positions.len() || c.uv.is_some_and(|t| t >= s.uvs.len()) {
                return Err(LineError {
                    line: face.line,
                    reason: "an index refers past the last vertex or texture coordinate".into(),
                });
            }
        }
    }
    Ok(())
}

fn refuse_non_finite(s: &Statements) -> Result<(), IoError> {
    let vertices: Vec<SourceCorner> = s
        .positions
        .iter()
        .zip(&s.position_primitives)
        .enumerate()
        .filter(|(_, (p, _))| !p.is_finite())
        .map(|(i, (_, &primitive))| SourceCorner {
            primitive,
            // Lossless: the vertex count fits in u32 (checked in `read`).
            vertex: i as u32,
        })
        .collect();
    if vertices.is_empty() {
        Ok(())
    } else {
        Err(IoError::NonFiniteInput { vertices })
    }
}

fn build(s: Statements) -> Result<TriangleSoup, IoError> {
    let has_uvs = s
        .faces
        .iter()
        .flat_map(|f| &f.corners)
        .any(|c| c.uv.is_some());
    let mut triangles = Vec::new();
    let mut sources = Vec::new();
    let mut uvs: Vec<Option<DVec2>> = Vec::new();
    // INVARIANT: `check_ranges` passed, so every corner's vertex and UV index is in
    // range, and `triangulate` returns indices below the face's corner count.
    for face in &s.faces {
        let split = split(face, &s).ok_or_else(|| {
            located(LineError {
                line: face.line,
                reason: "the polygon crosses itself, so it has no triangulation".into(),
            })
        })?;
        for tri in split {
            for c in tri.map(|i| face.corners[i]) {
                // Lossless: indices are below the vertex count, which fits in u32.
                let vertex = c.vertex as u32;
                sources.push(SourceCorner {
                    primitive: face.primitive,
                    vertex,
                });
                uvs.push(c.uv.map(|t| s.uvs[t]));
            }
            let [a, b, c] = tri.map(|i| face.corners[i].vertex as u32);
            triangles.push([a, b, c]);
        }
    }
    let uvs = has_uvs.then_some(uvs);
    let materials = SourceMaterials::new(
        s.material_names,
        s.primitive_materials
            .iter()
            // Lossless: a name is added only for a primitive, and primitives fit in u32.
            .map(|m| m.map(|i| MaterialIndex::new(i as u32)))
            .collect(),
    );
    // No other UV set exists: OBJ carries only the active map.
    Ok(TriangleSoup::new(s.positions, triangles, sources, uvs)?.with_materials(materials, 0)?)
}

/// A face's triangles as indices into its corners.
fn split(face: &Face, s: &Statements) -> Option<Vec<[usize; 3]>> {
    // INVARIANT: `check_ranges` passed, so every corner's vertex is in range.
    let points: Vec<_> = face.corners.iter().map(|c| s.positions[c.vertex]).collect();
    triangulate(&points)
}
