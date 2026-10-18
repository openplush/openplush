//! PLY geometry, ASCII or binary little or big endian.
//!
//! The `vertex` element gives the soup's vertices in file order, from its `x`, `y` and
//! `z` and, if it has both, a texture coordinate pair (`s`/`t`, `u`/`v`, `texture_s`/
//! `texture_t` or `texture_u`/`texture_v`), which each face corner copies from its
//! vertex. The `face` element's `vertex_indices` (or `vertex_index`) list gives the
//! polygons, triangulated. A corner's provenance is primitive 0 and its vertex position.
//! Every other property and element is read past and ignored.

mod elements;
mod header;
mod source;

use openplush_geom::DVec3;
use openplush_mesh::{SourceCorner, SourceMaterials, TriangleSoup};

use crate::triangulate::triangulate;
use crate::{Format, IoError};
use elements::Vertices;
use header::{Header, Kind};
use source::Source;

/// Why the file could not be read, in the reader's words.
#[derive(Debug)]
struct Failure(String);

impl From<Failure> for IoError {
    fn from(Failure(detail): Failure) -> Self {
        IoError::Parse {
            format: Format::Ply,
            detail,
        }
    }
}

/// Reads PLY bytes into a soup, triangulating polygons.
///
/// Errors: [`IoError::Parse`] for a header or body it cannot read (the detail starts
/// `line N:` or `byte N:` where it can say where), a face with fewer than three corners
/// or an index past the last vertex, or a polygon that crosses itself;
/// [`IoError::NonFiniteInput`] naming every vertex with a NaN or infinite coordinate.
pub fn read(bytes: &[u8]) -> Result<TriangleSoup, IoError> {
    let header = header::parse(bytes)?;
    check_elements(&header)?;
    let mut source = Source::new(header.encoding, bytes, header.body, header.body_line)?;
    let mut vertices = None;
    let mut faces = Vec::new();
    for element in &header.elements {
        match element.name.as_str() {
            "vertex" => vertices = Some(elements::read_vertices(element, &mut source)?),
            "face" => {
                // INVARIANT: `check_elements` put `vertex` before `face`.
                let count = vertices
                    .as_ref()
                    .map_or(0, |v: &Vertices| v.positions.len());
                faces = elements::read_faces(element, &mut source, count)?;
            }
            _ => elements::skip(element, &mut source)?,
        }
    }
    // INVARIANT: `check_elements` found a `vertex` element, so it was read.
    let vertices = vertices.ok_or_else(|| Failure("the file has no `vertex` element".into()))?;
    build(vertices, &faces)
}

fn check_elements(header: &Header) -> Result<(), Failure> {
    let position = |name: &str| header.elements.iter().position(|e| e.name == name);
    let vertex =
        position("vertex").ok_or_else(|| Failure("the file has no `vertex` element".into()))?;
    let has = |name: &str| {
        header.elements[vertex]
            .properties
            .iter()
            .any(|p| p.name == name && matches!(p.kind, Kind::Scalar(_)))
    };
    if !(has("x") && has("y") && has("z")) {
        return Err(Failure(
            "the `vertex` element needs scalar `x`, `y` and `z`".into(),
        ));
    }
    if position("face").is_some_and(|face| face < vertex) {
        return Err(Failure(
            "the `face` element comes before the `vertex` element".into(),
        ));
    }
    Ok(())
}

fn build(vertices: Vertices, faces: &[Vec<u32>]) -> Result<TriangleSoup, IoError> {
    let Vertices { positions, uvs } = vertices;
    let Ok(count) = u32::try_from(positions.len()) else {
        return Err(Failure(format!(
            "{} vertices exceed the u32 index space",
            positions.len()
        ))
        .into());
    };
    let non_finite: Vec<SourceCorner> = positions
        .iter()
        .zip(0..count)
        .filter(|(p, _)| !p.is_finite())
        .map(|(_, vertex)| SourceCorner {
            primitive: 0,
            vertex,
        })
        .collect();
    if !non_finite.is_empty() {
        return Err(IoError::NonFiniteInput {
            vertices: non_finite,
        });
    }
    let mut triangles = Vec::new();
    let mut sources = Vec::new();
    let mut corner_uvs = Vec::new();
    for (face, corners) in faces.iter().enumerate() {
        let points: Vec<DVec3> = corners.iter().map(|&i| positions[i as usize]).collect();
        let split = triangulate(&points).ok_or_else(|| {
            Failure(format!(
                "face {face} crosses itself, so it has no triangulation"
            ))
        })?;
        for tri in split {
            let corners = tri.map(|i| corners[i]);
            triangles.push(corners);
            for vertex in corners {
                sources.push(SourceCorner {
                    primitive: 0,
                    vertex,
                });
                corner_uvs.push(uvs.as_ref().map(|u| u[vertex as usize]));
            }
        }
    }
    let corner_uvs = uvs.is_some().then_some(corner_uvs);
    let materials = SourceMaterials::new(Vec::new(), vec![None]);
    Ok(
        TriangleSoup::new(positions, triangles, sources, corner_uvs)?
            .with_materials(materials, 0)?,
    )
}
